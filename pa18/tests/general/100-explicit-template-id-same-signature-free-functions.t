template<class T>
int f() { return sizeof(T); }

int g() { return f<char>() + f<long>(); }
