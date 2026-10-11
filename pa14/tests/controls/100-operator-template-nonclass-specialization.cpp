// N3485 13.5p6, 14.8.2p8: considering template<class T> int operator*(T,
// double) with T=int for operator*<int>(w, 2) is not a hard error (the
// candidate is simply not viable); N::operator*<int> via ADL is chosen;
// expect exit 0
namespace N { struct W { int y; }; template<class T> int operator*(W w, T t) { return w.y * static_cast<int>(t); } }
template<class T> int operator*(T, double) { return 100; }
int main() { N::W w = { 3 }; return operator*<int>(w, 2) == 6 ? 0 : 1; }
