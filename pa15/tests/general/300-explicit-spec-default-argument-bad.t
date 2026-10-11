// N3485 14.7.3p19 (C++11 14.7.3p21): default function arguments shall not be
// specified in an explicit specialization of a function template; expect
// reject
template<class T> int f(T, int = 1) { return 0; }
template<> int f<char>(char, int = 2) { return 0; }
int main() { return 0; }
