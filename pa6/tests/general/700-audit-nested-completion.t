// Frozen PA6 types-mode scope/declaration ownership regression.
struct C{void f(){struct L{void g(){Word n;}typedef int Word;};}void h(){Unknown n;}};
