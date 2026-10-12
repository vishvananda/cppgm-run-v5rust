// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
void f() noexcept(sizeof(f())>0);
