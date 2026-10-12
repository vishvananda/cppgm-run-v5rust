// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
void f(int a()) noexcept(sizeof(decltype(a))==8);
