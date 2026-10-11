// N3485 focus: 14.8.2.1 [temp.deduct.call], 14.5.3 [temp.variadic]
template<class T, class C, class... A>
int call(T (*)(C*, A...), C*, A...) { return 0; }

long convert(char*, int) { return 0; }

int main() { return call<long, char>(&convert, (char*)0, 1); }
