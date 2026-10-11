// N3485 14.8.2.5p9 (pack expansion pattern T*... deduced from each argument):
// f(X<int*, char*, long*>()) deduces T = {int, char, long}; expect exit 0
template<class... T> struct X {};
template<class... T> int f(X<T*...>) { return (int)sizeof...(T); }
int main() { return f(X<int*, char*, long*>()) == 3 ? 0 : 1; }
