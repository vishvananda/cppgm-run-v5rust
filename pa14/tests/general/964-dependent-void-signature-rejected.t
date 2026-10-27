template<class T> struct A { using F = void(T); };
A<void>::F *object;
int main() { return 0; }
