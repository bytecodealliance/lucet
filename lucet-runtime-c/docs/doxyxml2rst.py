import xml.etree.ElementTree as ET
import re
from jinja2 import Template, Environment, FileSystemLoader

def parse_index():
    file_index = []
    tree = ET.parse('doxyxml/index.xml')
    root = tree.getroot()

    for n in root.iter('compound'):
        if n.attrib['kind'] == 'file':
            name = n.find('name')
            file_index.append((name.text, n.attrib['refid']))

    return file_index

def convert_functions(n, data):
    for func in n.iter('memberdef'):
        args = ",".join(''.join(param.find('type').itertext()) for param in func.iter('param'))
        prototype = func.find('definition').text + "(" + args + ")"

        brief, details = extract_desc(func)
        data['functions'].append({ 'prototype': prototype, 'brief': brief, 'details': details })

def convert_enums(n, data):
    for enum in n.iter('memberdef'):
        definition = "enum %s" % enum.find('name').text
        brief, details = extract_desc(enum)
        vals = []
        for val in enum.iter('enumvalue'):
            val_name = val.find('name').text
            val_brief, val_details = extract_desc(val)
            vals.append({ 'name': val_name, 'brief': val_brief, 'details': val_details })

        data['types'].append({ 'definition': definition, 'brief': brief, 'details': details, 'vals': vals })

def convert_typedefs(n, data):
    for typedef in n.iter('memberdef'):
        definition = typedef.find('definition').text
        brief, details = extract_desc(typedef)
        data['types'].append({ 'definition': definition, 'brief': brief, 'details': details })

def convert_defines(n, data):
    for define in n.iter('memberdef'):
        if define.find('initializer') is None:
            # Don't bother displaying defines that don't have an initializer
            continue

        definition = "#define %s %s" % (define.find('name').text, define.find('initializer').text)
        brief, details = extract_desc(define)
        data['defines'].append({ 'definition': definition, 'brief': brief, 'details': details })

def convert_struct(n, data):
    for cdef in n.iter('compounddef'):
        definition = 'struct ' + cdef.find('compoundname').text
        brief, details = extract_desc(cdef)
        members = []
        for mdef in cdef.iter('memberdef'):
            mdef_brief, mdef_details = extract_desc(mdef)
            mdef_def = mdef.find('definition').text
            if mdef.find('argsstring').text is not None:
                mdef_def = mdef_def.replace(mdef.find('argsstring').text, '')
            
            member = { 'definition': mdef_def,
                       'name': mdef.find('name').text,
                       'brief': mdef_brief,
                       'details': mdef_details }
            members.append(member)
            
        data['types'].append({ 'definition': definition, 'brief': brief, 'details': details, 'members': members })

def extract_desc(n):
    brief = n.find('briefdescription').text.strip()
    details = []
    for para in n.find('detaileddescription').iter('para'):
        para_text = ''
        for text in para.itertext():
            para_text += text
        details.append(para_text.strip())

    if brief == '' and len(details) > 0:
        brief = details.pop(0)
    return (brief, details)

        
def convert_file(infile):
    data = { 'functions': [], 'types': [], 'defines': [] }
    tree = ET.parse(infile)
    root = tree.getroot()
    for n in root.iter('sectiondef'):
        if n.attrib['kind'] == 'func':
            convert_functions(n, data)
        elif n.attrib['kind'] == 'enum':
            convert_enums(n, data)
        elif n.attrib['kind'] == 'typedef':
            convert_typedefs(n, data)
        elif n.attrib['kind'] == 'define':
            convert_defines(n, data)

    for n in root.iter('innerclass'):
        structtree = ET.parse('doxyxml/%s.xml' % n.attrib['refid'])
        convert_struct(structtree.getroot(), data)

    return data
            

file_index = parse_index()
for (filename, refid) in file_index:
    data = convert_file("doxyxml/%s.xml" % refid)
    
    env = Environment(loader=FileSystemLoader('./'))
    template = env.get_template('api.rst.in')
    output = template.render(headerfile=filename,
                             functions=data['functions'],
                             types=data['types'],
                             defines=data['defines'])

    with open("_build/%s.rst" % filename, 'w') as f:
        f.write(output)
        
