template <class T> struct Tag { static int id; static int count(); };
template <> struct Tag<char> { static int id; static int count(); };
int Tag<char>::id = 7;
int Tag<char>::count() { return id * 2; }
