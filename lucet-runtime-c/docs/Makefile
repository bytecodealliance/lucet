# Minimal makefile for Sphinx documentation
#

# You can set these variables from the command line.
SPHINXOPTS    =
SPHINXBUILD   = sphinx-build
SPHINXPROJ    = liblucet-runtime-c
SOURCEDIR     = .
BUILDDIR      = _build

# Put it first so that "make" without argument is like "make help".
help:
	@$(SPHINXBUILD) -M help "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)

.PHONY: help Makefile

.PHONY: clean
clean:
	rm -rf _build

.PHONY: html
html:
	@mkdir -p _build
	doxygen doxygen.conf
	python doxyxml2rst.py
	$(SPHINXBUILD) -Q -b html "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
	@echo "\033[0;32mOpen lucet-runtime-c/docs/_build/index.html in a browser to view documentation!\033[0m"

.PHONY: debug-html
debug-html:
	@mkdir -p _build
	( cat doxygen.conf ; echo "QUIET = NO" ) | doxygen -
	python doxyxml2rst.py
	$(SPHINXBUILD) -b html "$(SOURCEDIR)" "$(BUILDDIR)" $(SPHINXOPTS) $(O)
