struct X { void f(int &); void f(int, int &); };

template<class R, class A, class B, class C, class D>
void bind(R (*)(A, B), C, D);

template<class R, class T, class A, class B, class C>
int bind(R (T::*)(A), B, C);

X x;
int i;

int main() { return bind(&X::f, &x, i); }
