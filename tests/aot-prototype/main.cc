// LLVM
#include "llvm/ADT/APFloat.h"
#include "llvm/ADT/Optional.h"
#include "llvm/ADT/STLExtras.h"
#include "llvm/IR/BasicBlock.h"
#include "llvm/IR/Constants.h"
#include "llvm/IR/DerivedTypes.h"
#include "llvm/IR/Function.h"
#include "llvm/IR/Instructions.h"
#include "llvm/IR/IRBuilder.h"
#include "llvm/IR/LLVMContext.h"
#include "llvm/IR/LegacyPassManager.h"
#include "llvm/IR/Module.h"
#include "llvm/IR/Type.h"
#include "llvm/IR/Verifier.h"
#include "llvm/Support/FileSystem.h"
#include "llvm/Support/Host.h"
#include "llvm/Support/raw_ostream.h"
#include "llvm/Support/TargetRegistry.h"
#include "llvm/Support/TargetSelect.h"
#include "llvm/Target/TargetMachine.h"
#include "llvm/Target/TargetOptions.h"
#include "llvm/Bitcode/BitcodeWriter.h"

// WABT
#include <binary-reader.h>
#include <binary-reader-nop.h>


using namespace llvm;
using namespace wabt;

struct ControlFrame {
	llvm::BasicBlock *entry;
	llvm::BasicBlock *exit;
	llvm::BasicBlock *br_target; // points to entry for loops, exit for blocks
};

struct dataSegment {
	const void *data;
	Address size;
	uint32_t loc;
};


class Generator : public BinaryReaderNop {
public:
	bool OnError(const char* message) override;
	Result EndModule() override;	
	Result OnFunctionCount(Index count) override;
	Result OnFunction(Index index, Index sig_index) override;
	Result BeginFunctionBody(Index index) override;
	Result EndFunctionBody(Index index) override;
	Result OnFunctionNamesCount(Index num_functions) override;
	Result OnFunctionName(Index function_index, StringSlice function_name) override;
	Result OnTypeCount(Index count) override;
	Result OnType(Index index,
	    Index param_count,
	    wabt::Type* param_types,
	    Index result_count,
	    wabt::Type* result_types) override;
	Result OnOpcode(Opcode opcode) override;
	Result OnBlockExpr(Index num_types, wabt::Type* sig_types) override;
	Result OnI32ConstExpr(uint32_t value) override;
	Result OnI64ConstExpr(uint64_t value) override;
	Result OnLoadExpr(Opcode opcode,
	    uint32_t alignment_log2,
	    Address offset) override;
	Result OnStoreExpr(Opcode opcode, uint32_t alignment_log2, Address offset) override;
	Result OnBinaryExpr(Opcode opcode) override;
	Result OnGetLocalExpr(Index local_index) override;
	Result OnTeeLocalExpr(Index local_index) override;
	Result OnSetLocalExpr(Index local_index) override;
	Result OnLocalDeclCount(Index count) override;
	Result OnLocalDecl(Index decl_index, Index count, wabt::Type type) override;
	Result OnDropExpr() override;
	Result OnCallExpr(Index func_index) override;
	Result OnImportFunc(Index import_index,
	    StringSlice module_name,
	    StringSlice field_name,
	    Index func_index,
	    Index sig_index) override;
	Result OnEndExpr() override;
	Result OnLoopExpr(Index num_types, wabt::Type* sig_types) override;
	Result OnCompareExpr(Opcode opcode) override;
	Result OnBrIfExpr(Index depth) override;
	Result OnCallIndirectExpr(Index sig_index) override;
	Result OnTableCount(Index count) override;
	Result OnTable(Index index,
	    wabt::Type elem_type,
	    const Limits* elem_limits) override;
	Result EndTableSection() override;
	Result BeginDataSection(Offset size) override;
	Result OnDataSegmentCount(Index count) override;
	Result OnDataSegmentData(Index index,
	    const void* data,
	    Address size) override;
	Result EndDataSection() override;
	Result BeginDataSegmentInitExpr(Index index) override;
	Result OnInitExprI32ConstExpr(Index index, uint32_t value) override;
	Result EndDataSegmentInitExpr(Index index) override;

	
	LLVMContext Context;
	llvm::Function* CurrentFunction;
	IRBuilder<> *Builder;
	std::unique_ptr<Module> Mod;

private:
	llvm::Value* pop();
	llvm::Value* peek();
	void push(llvm::Value*);
	llvm::Value* calculateEffectiveAddr(llvm::Value *base, Address offset);

	bool inDataSection;
	std::vector<struct dataSegment> dataSegments;
	llvm::Type* TableElementType;
	llvm::Type* InstanceType;
	std::vector<char *> FunctionNames;
	std::vector<FunctionType*> TypeSigs;
	std::vector<Index> FunctionTypes;
	std::vector<Function*> Functions;
	std::vector<llvm::Value*> ValueStack;
	std::vector<llvm::Value*> Locals;
	std::vector<ControlFrame> ControlStack;
	uint64_t TableSize;
};


Result Generator::BeginDataSection(Offset size) {
	inDataSection = true;
       	return Result::Ok;
}

Result Generator::OnDataSegmentCount(Index count) {
	printf("OnDataSegmentCount: %u\n", count);
	dataSegments.resize(count);
       	return Result::Ok;
}

Result Generator::BeginDataSegmentInitExpr(Index index) {
	return Result::Ok;
}

Result Generator::OnInitExprI32ConstExpr(Index index, uint32_t value) {
	if (!inDataSection) {
		return Result::Ok;
	}
	printf("OnInitExprI32ConstExpr: %u %u\n", index, value);
	dataSegments[index].loc = value;
	return Result::Ok;
}

Result Generator::EndDataSegmentInitExpr(Index index) {
	return Result::Ok;
}

Result Generator::OnDataSegmentData(Index index,
    const void* data,
    Address size) {
	printf("OnDataSegmentData: index=%u, size=%u\n", index, size);

	dataSegments[index].data = data;
	dataSegments[index].size = size;
	
	return Result::Ok;
}

Result Generator::EndDataSection() {
	inDataSection = false;

	unsigned size = 0;
	
	for (auto segment : dataSegments) {
		unsigned reach = segment.loc + segment.size;
		size = reach > size ? reach : size;
	}
	
	char *data = (char *)calloc(1, size);
	assert(data);

	for (auto segment : dataSegments) {
		char *curs = &data[segment.loc];
		memcpy(curs, segment.data, segment.size);
	}

	llvm::Constant* array = ConstantDataArray::get(Context, ArrayRef<uint8_t>((uint8_t*) data, size));


	auto type = llvm::ArrayType::get(llvm::Type::getInt8Ty(Context), size);
	Mod->getOrInsertGlobal("data_segments", type);
	auto var = Mod->getNamedGlobal("data_segments");
	var->setLinkage(llvm::GlobalValue::ExternalLinkage);
	var->setInitializer(array);

	Mod->getOrInsertGlobal("data_size", llvm::Type::getInt32Ty(Context));
	auto dataSize = Mod->getNamedGlobal("data_size");
	dataSize->setLinkage(llvm::GlobalValue::ExternalLinkage);
	dataSize->setInitializer(ConstantInt::get(llvm::Type::getInt32Ty(Context), size));

	return Result::Ok;
}

Result Generator::EndTableSection() {
	ArrayRef<llvm::Type*> tableFields = {
		llvm::Type::getInt32Ty(Context),   // type
		llvm::Type::getInt8PtrTy(Context), // function ptr
	};
	TableElementType = llvm::StructType::create(Context, tableFields, "TableElement");
	auto tableArray = llvm::ArrayType::get(TableElementType, TableSize);

	auto byte = llvm::Type::getInt8Ty(Context);
	auto memory = llvm::ArrayType::get(byte, 1);
	
	ArrayRef<llvm::Type*> instanceFields = {
		tableArray,
		memory,
	};
	
	InstanceType = llvm::StructType::create(Context, instanceFields, "InstanceType")->getPointerTo();

	return Result::Ok;
}

bool Generator::OnError(const char* message) {
	fputs(message, stderr);
	return false;
}

Result Generator::OnImportFunc(Index import_index,
    StringSlice module_name,
    StringSlice field_name,
    Index func_index,
    Index sig_index) {
	if (FunctionTypes.size() < func_index + 1) {
		FunctionTypes.resize(func_index + 1);
	}
	FunctionTypes[func_index] = sig_index;
	FunctionType *FT = TypeSigs[sig_index];

	char name[128];
	memcpy(name, module_name.start, module_name.length);
	name[module_name.length] = '_';
	memcpy(&name[module_name.length + 1], field_name.start, field_name.length);
	name[module_name.length + field_name.length + 1] = '\0';
	
	if (Functions.size() < func_index + 1) {
		Functions.resize(func_index + 1);
	}
	Functions[func_index] = Function::Create(FT, Function::ExternalLinkage, name, Mod.get());
	
	return Result::Ok;
}

Result Generator::OnTypeCount(Index count) {
	puts("OnTypeCount");
	TypeSigs.resize(count);
	return Result::Ok;
}

Result Generator::OnType(Index index, Index param_count, wabt::Type* param_types, Index result_count, wabt::Type* result_types) {
	assert(result_count <= 1);

	std::vector<llvm::Type*> arg_types = { llvm::Type::getInt8PtrTy(Context) };
	for (Index i = 0; i < param_count; i++) {
		arg_types.push_back(llvm::Type::getInt32Ty(Context));
	}
	
	llvm::Type *return_type;
	if (result_count > 0) {
		return_type = llvm::Type::getInt32Ty(Context);
	} else {
		return_type = llvm::Type::getVoidTy(Context);
	}

	TypeSigs[index] = FunctionType::get(return_type, arg_types, 0);
	return Result::Ok;
}

Result Generator::OnFunctionNamesCount(Index num_functions) {
	FunctionNames.resize(num_functions);
	return Result::Ok;
}
Result Generator::OnFunctionName(Index idx, StringSlice name) {
	FunctionNames[idx] = (char *) calloc(1, name.length+1);
	strncpy(FunctionNames[idx], name.start, name.length);
	printf("Got function name %s for index %d\n", FunctionNames[idx], idx);
	return Result::Ok;
}

Result Generator::OnFunction(Index index, Index sig_index) {
	char name[128];
	FunctionType *FT = TypeSigs[sig_index];

	FunctionTypes[index] = sig_index;

	snprintf(name, 128, "function_%d", index);
	
	Function *F = Function::Create(FT, Function::ExternalLinkage, name, Mod.get());
	Functions[index] = F;

	return Result::Ok;
}


Result Generator::OnFunctionCount(Index count) {
	printf("This module has %d functions\n", count);
	FunctionTypes.resize(count);
	Functions.resize(count);
	return Result::Ok;
}

Result Generator::BeginFunctionBody(Index index) {
	CurrentFunction = Functions[index];

	printf("Beginning function body %d\n", index);


	ControlFrame controlFrame;
	controlFrame.entry = BasicBlock::Create(Context, "entry", CurrentFunction);
	controlFrame.exit = BasicBlock::Create(Context, "exit", CurrentFunction);
	controlFrame.br_target = controlFrame.exit;

	ControlStack.push_back(controlFrame);

	Builder->SetInsertPoint(controlFrame.entry);

	Locals.resize(CurrentFunction->arg_size()-1);
	for (auto &Arg : CurrentFunction->args()) {
		if (Arg.getArgNo() == 0) {
			continue;
		}
		
		// Create an alloca for this variable.
		AllocaInst *Alloca = Builder->CreateAlloca(Arg.getType(), 0);
		
		// Store the initial value into the alloca.
		Builder->CreateStore(&Arg, Alloca);
		
		// Add arguments to variable symbol table.
		Locals[Arg.getArgNo()-1] = Alloca;
	}

	
	return Result::Ok;
}

Result Generator::OnOpcode(Opcode opcode) {
	if (ValueStack.size() > 0
	    && ValueStack.back()
	    && ValueStack.back()->getType()->isPointerTy()) {
		printf("Found pointer on the stack!\n");
		peek()->dump();
		assert(false);
	}
	printf("  %s\n", get_opcode_name(opcode));
	return Result::Ok;
}

Result Generator::OnBlockExpr(Index num_types, wabt::Type* sig_types) {
	ControlFrame controlFrame;
	controlFrame.entry = BasicBlock::Create(Context, "block_start", CurrentFunction);
	controlFrame.exit = BasicBlock::Create(Context, "block_end", CurrentFunction);
	controlFrame.br_target = controlFrame.exit;

	ControlStack.push_back(controlFrame);
	Builder->CreateBr(controlFrame.entry);
	Builder->SetInsertPoint(controlFrame.entry);

	return Result::Ok;
}

Result Generator::OnLoopExpr(Index num_types, wabt::Type* sig_types) {
	ControlFrame controlFrame;
	controlFrame.entry = BasicBlock::Create(Context, "loop_start", CurrentFunction);
	controlFrame.exit = BasicBlock::Create(Context, "loop_end", CurrentFunction);
	controlFrame.br_target = controlFrame.entry;

	ControlStack.push_back(controlFrame);
	Builder->CreateBr(controlFrame.entry);
	Builder->SetInsertPoint(controlFrame.entry);

	return Result::Ok;
}

Result Generator::OnEndExpr() {
	ControlFrame controlFrame = ControlStack.back();
	ControlStack.pop_back();

	Builder->CreateBr(controlFrame.exit);
	controlFrame.exit->moveAfter(Builder->GetInsertBlock());
	Builder->SetInsertPoint(controlFrame.exit);

	return Result::Ok;
}

Result Generator::OnCompareExpr(Opcode opcode) {
	auto rhs = pop();
	auto lhs = pop();
	
	switch (opcode) {
	case Opcode::I32GeS:
		push(Builder->CreateICmpSGE(lhs, rhs));
		break;
	default:
		printf("OnCompareExpr: Unexpected opcode %s\n", get_opcode_name(opcode));
		return Result::Error;
	}
	return Result::Ok;
}

Result Generator::OnBrIfExpr(Index depth) {
	printf("OnBrIfExpr: depth=%u\n", depth);
	auto value = pop();

	auto stackIdx = ControlStack.size() - depth - 1;
	auto targetFrame = ControlStack[stackIdx];

	auto brFalse = BasicBlock::Create(Context, "br_false", CurrentFunction);
	
	Builder->CreateCondBr(value, targetFrame.br_target, brFalse);
	Builder->SetInsertPoint(brFalse);
	
	return Result::Ok;
}



Result Generator::OnI32ConstExpr(uint32_t value) {
	llvm::Type *type = llvm::Type::getInt32Ty(Context);
	llvm::Value *expr = llvm::ConstantInt::get(type, value);
	push(expr);

	return Result::Ok;
}
Result Generator::OnI64ConstExpr(uint64_t value) {
	llvm::Type *type = llvm::Type::getInt64Ty(Context);
	llvm::Value *expr = llvm::ConstantInt::get(type, value);
	push(expr);
	return Result::Ok;
}

Result Generator::OnLoadExpr(Opcode opcode, uint32_t alignment_log2, Address offset) {
	llvm::Value *effectiveAddr = calculateEffectiveAddr(pop(), offset);

	llvm::Type *loadType = nullptr;
	switch (opcode) {
	case Opcode::I32Load:
		loadType = llvm::Type::getInt32Ty(Context);
		break;
	default:
		assert(false);
		return Result::Error;
	}

	llvm::Value *pointer = Builder->CreatePointerCast(effectiveAddr, loadType->getPointerTo());

	push(Builder->CreateLoad(pointer));
	return Result::Ok;

}
Result Generator::OnStoreExpr(Opcode opcode, uint32_t alignment_log2, Address offset) {
	llvm::Value *value = pop();
	llvm::Value *idx = pop();

	llvm::Value *effectiveAddr = calculateEffectiveAddr(idx, offset);

	llvm::Type *loadType = nullptr;
	switch (opcode) {
	case Opcode::I32Store:
		loadType = llvm::Type::getInt32Ty(Context);
		break;
	default:
		assert(false);
		return Result::Error;
	}

	llvm::Value *pointer = Builder->CreatePointerCast(effectiveAddr, loadType->getPointerTo());
	assert(value != NULL);
	Builder->CreateStore(value, pointer);
	
	return Result::Ok;

}

llvm::Value* Generator::calculateEffectiveAddr(llvm::Value *base, Address offset) {
	auto instanceHandle = &*CurrentFunction->arg_begin();
	auto instance = Builder->CreatePointerCast(instanceHandle, InstanceType);

	auto offsetIdx = ConstantInt::get(llvm::Type::getInt32Ty(Context), offset);
	auto index = Builder->CreateAdd(base, offsetIdx);
	auto effAddr = Builder->CreateZExt(index, llvm::Type::getInt64Ty(Context));

	// skip over table field
	auto throughPtr = ConstantInt::get(llvm::Type::getInt32Ty(Context), 0);
	auto fieldIdx = ConstantInt::get(llvm::Type::getInt32Ty(Context), 1);

	auto gep = Builder->CreateInBoundsGEP(instance, { throughPtr, fieldIdx, effAddr });

	return gep;
}

Result Generator::OnBinaryExpr(Opcode opcode) {
	auto rhs = pop();
	auto lhs = pop();

	switch (opcode) {
	case Opcode::I32Sub:
		push(Builder->CreateSub(lhs, rhs));
		break;
	case Opcode::I32Add:
		push(Builder->CreateAdd(lhs, rhs));
		break;
	case Opcode::I32Shl:
		push(Builder->CreateShl(lhs, rhs));
		break;
	default:
		return Result::Error;
	}
	
	return Result::Ok;
}

Result Generator::OnGetLocalExpr(Index local_index) {
	llvm::Value *load = Builder->CreateLoad(Locals[local_index]);
	push(load);
	return Result::Ok;
}
Result Generator::OnTeeLocalExpr(Index local_index) {
	llvm::Value *value = peek();
	Builder->CreateStore(value, Locals[local_index]);
	return Result::Ok;
}
Result Generator::OnSetLocalExpr(Index local_index) {
	llvm::Value *value = pop();
	Builder->CreateStore(value, Locals[local_index]);
	return Result::Ok;
}

Result Generator::EndFunctionBody(Index index) {
	ControlFrame controlFrame = ControlStack.back();
	ControlStack.pop_back();

	Builder->CreateBr(controlFrame.exit);
	controlFrame.exit->moveAfter(Builder->GetInsertBlock());
	Builder->SetInsertPoint(controlFrame.exit);
	
	if (CurrentFunction->getReturnType()->isVoidTy()) {
		Builder->CreateRetVoid();
	} else {
		Builder->CreateRet(pop());
	}
	CurrentFunction = nullptr;
	ControlStack.clear();
	
	return Result::Ok;
}

Result Generator::OnLocalDeclCount(Index count) {
	Locals.resize((CurrentFunction->arg_size() - 1) + count);
	
	return Result::Ok;
}
Result Generator::OnLocalDecl(Index decl_index, Index count, wabt::Type type) {
	llvm::Type *ltype = nullptr;
	llvm::Value *expr = nullptr;
	switch (type) {
	case wabt::Type::I32:
		ltype = llvm::Type::getInt32Ty(Context);
		expr = llvm::ConstantInt::get(ltype, 0);
		break;
	default:
		return Result::Error;
	}

	AllocaInst *Alloca = Builder->CreateAlloca(ltype, 0);
	Builder->CreateStore(expr, Alloca);
	Locals[(CurrentFunction->arg_size() - 1) + decl_index] = Alloca;

	return Result::Ok;
}

Result Generator::OnDropExpr() {
	pop();
	return Result::Ok;
}

Result Generator::OnCallExpr(Index func_index) {
	Index typeIdx = FunctionTypes[func_index];
	FunctionType* ftype = TypeSigs[typeIdx];
	Function* callee = Functions[func_index];

	std::vector<llvm::Value*> args;
	// state pointer
	args.push_back(&*CurrentFunction->arg_begin());

	for (unsigned i = 0; i < ftype->getNumParams()-1; i++) {
		args.push_back(pop());
	}
	
	llvm::Value *call = Builder->CreateCall(callee, ArrayRef<llvm::Value*>(args));

	if (!ftype->getReturnType()->isVoidTy()) {
		push(call);
	}
	return Result::Ok;
}

Result Generator::OnCallIndirectExpr(Index sig_index) {
	// cast the instance pointer
	auto instanceHandle = &*CurrentFunction->arg_begin();
	auto instance = Builder->CreatePointerCast(instanceHandle, InstanceType);

	// get the table element
	auto throughPtr = ConstantInt::get(llvm::Type::getInt32Ty(Context), 0);
	auto fieldIdx = ConstantInt::get(llvm::Type::getInt32Ty(Context), 0);
	auto tableIdx = pop();
	auto tableElement = Builder->CreateInBoundsGEP(instance, { throughPtr, fieldIdx, tableIdx });

	// check the type signature
	auto sigField = Builder->CreateInBoundsGEP(tableElement, { ConstantInt::get(llvm::Type::getInt32Ty(Context), 0), ConstantInt::get(llvm::Type::getInt32Ty(Context), 0) }); // first 0 to step through the pointer, the second to indicate the first field of the table element
	
	auto actualType = Builder->CreateLoad(sigField);
	auto expectedType = ConstantInt::get(llvm::Type::getInt32Ty(Context), sig_index);
	auto typeCmp = Builder->CreateICmpEQ(actualType, expectedType);

	auto brTrue = BasicBlock::Create(Context, "call_indirect_match", CurrentFunction);
	//TODO: one per function
	auto brFalse = BasicBlock::Create(Context, "call_indirect_mismatch", CurrentFunction);
	Builder->CreateCondBr(typeCmp, brTrue, brFalse);
	
	Builder->SetInsertPoint(brFalse);
	Builder->CreateUnreachable();

	Builder->SetInsertPoint(brTrue);

	// ok, now the call
	auto ftype = TypeSigs[sig_index];

	std::vector<llvm::Value*> args;
	args.push_back(&*CurrentFunction->arg_begin());
	for (unsigned i = 0; i < ftype->getNumParams()-1; i++) {
		args.push_back(pop());
	}

	auto functionFieldHandle = Builder->CreateInBoundsGEP(tableElement, { ConstantInt::get(llvm::Type::getInt32Ty(Context), 0), ConstantInt::get(llvm::Type::getInt32Ty(Context), 1) });
	functionFieldHandle->getType()->dump();

	auto functionPtr = Builder->CreateLoad(Builder->CreatePointerCast(functionFieldHandle, ftype->getPointerTo()->getPointerTo()));
	functionPtr->getType()->dump();
	
	auto call = Builder->CreateCall(functionPtr, ArrayRef<llvm::Value*>(args));

	if (!ftype->getReturnType()->isVoidTy()) {
		push(call);
	}
	
	return Result::Ok;
}

Result Generator::OnTableCount(Index count) {
	assert(count <= 1);
	return Result::Ok;
}

Result Generator::OnTable(Index index,
    wabt::Type elem_type,
    const Limits* elem_limits) {

	assert(index == 0);
	assert(elem_type == wabt::Type::AnyFunc);

	TableSize = elem_limits->initial;
	return Result::Ok;
}




Result Generator::EndModule() {
	puts("Parsed a module!");
	return Result::Ok;
}

llvm::Value* Generator::pop() {
	llvm::Value *out = ValueStack.back();
	ValueStack.pop_back();
	return out;
}

llvm::Value* Generator::peek() {
	llvm::Value *out = ValueStack.back();
	return out;
}

void Generator::push(llvm::Value *value) {
	ValueStack.push_back(value);
}


size_t readfile(const char *filename, char **outbuf) {
	FILE *fp;
	size_t size;
	char *buffer;

	fp = fopen(filename, "r");
	if(!fp) {
		perror(filename);
		exit(1);
	}

	fseek(fp, 0L, SEEK_END);
	size = ftell(fp);
	rewind(fp);

	/* allocate memory for entire content */
	buffer = (char *)calloc(1, size);
	if(!buffer) {
		fclose(fp);
		fputs("memory alloc fails",stderr);
		exit(1);
	}

	/* copy the file into the buffer */
	if(fread(buffer, size, 1, fp) != 1) {
		fclose(fp);
		free(buffer);
		fputs("entire read fails",stderr);
		exit(1);
	}

	fclose(fp);

	*outbuf = buffer;
	return size;
}

int main(int argc, char *argv[]) {
	char *buffer;
	size_t buffer_len;

	if (argc == 1) {
		return 1;
	}
	buffer_len = readfile(argv[1], &buffer);
	
	ReadBinaryOptions opts;
	opts.read_debug_names = false;
	opts.log_stream = NULL;

	auto reader = new Generator();
	reader->Builder = new IRBuilder<>(reader->Context);

	// LLVM Module setup
	reader->Mod = llvm::make_unique<Module>("test", reader->Context);


	// Read it
	read_binary(buffer, buffer_len, reader, &opts);

	// LLVM output setup
	InitializeAllTargetInfos();
	InitializeAllTargets();
	InitializeAllTargetMCs();
	InitializeAllAsmParsers();
	InitializeAllAsmPrinters();

	/*
	auto TargetTriple = sys::getDefaultTargetTriple();
	reader->Mod->setTargetTriple(TargetTriple);

	std::string Error;
	auto Target = TargetRegistry::lookupTarget(TargetTriple, Error);

	// Print an error and exit if we couldn't find the requested target.
	// This generally occurs if we've forgotten to initialise the
	// TargetRegistry or we have a bogus target triple.
	if (!Target) {
		errs() << Error;
		return 1;
	}

	auto CPU = "generic";
	auto Features = "";
	TargetOptions opt;
	auto RM = Optional<llvm::Reloc::Model>();
	auto TheTargetMachine =
	    Target->createTargetMachine(TargetTriple, CPU, Features, opt, RM);

	reader->Mod->setDataLayout(TheTargetMachine->createDataLayout());
	

	auto Filename = "output.o";
	std::error_code EC;
	raw_fd_ostream dest(Filename, EC, sys::fs::F_None);
	  
	if (EC) {
		errs() << "Could not open file: " << EC.message();
		return 1;
	}
	  
	legacy::PassManager pass;
	auto FileType = TargetMachine::CGFT_ObjectFile;
	  
	if (TheTargetMachine->addPassesToEmitFile(pass, dest, FileType)) {
		errs() << "TheTargetMachine can't emit a file of this type";
		return 1;
	}
	  
	pass.run(*reader->Mod);
	dest.flush();
	  
	outs() << "Wrote " << Filename << "\n";
	*/

	llvm::verifyModule(*(reader->Mod.get()), &errs());
	
	//reader->Mod->dump();
	
	auto Filename = "output.bc";
	std::error_code EC;
	llvm::raw_fd_ostream OS(Filename, EC, llvm::sys::fs::F_None);
	WriteBitcodeToFile(reader->Mod.get(), OS);
	OS.flush();
	
	return 0;
}
