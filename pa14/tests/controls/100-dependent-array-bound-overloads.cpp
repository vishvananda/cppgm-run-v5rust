// N3485 14.5.6.1p5-p6: function templates whose parameter types differ only in
// a dependent array bound (sizeof(T) vs sizeof(T) + 1, sizeof(T) vs
// alignof(T)) are distinct overloads, not redefinitions; expect exit 0
template<class T> int bound(char (&)[sizeof(T)]) { return 1; }
template<class T> int bound(char (&)[sizeof(T) + 1]) { return 2; }
int main() { char four[4], five[5]; return (bound<int>(four) == 1 && bound<int>(five) == 2) ? 0 : 1; }
