// N3485 14.5.6.2p3, 14.8.2.4p3 (address: the whole function type is ordered):
// T* f(int) is more specialized than T f(int) for target int*(*)(int);
// expect exit 0
int r = 0;
template<class T> T f(int) { r = 1; return T(); }
template<class T> T* f(int) { r = 2; return 0; }
int main() { int* (*s)(int) = f; s(0); return r == 2 ? 0 : 1; }
