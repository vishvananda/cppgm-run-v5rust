// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
void g(); void f() noexcept(true ? 1 : g());
