// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
struct C{ void f() throw(Missing); void g(){int a[0];} };
