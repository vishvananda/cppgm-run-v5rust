// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
void f(int a[3]) noexcept(sizeof(decltype(a))==8); void f(int a[3]) noexcept;
