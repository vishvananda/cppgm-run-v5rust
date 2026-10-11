struct X {};
template<class T> int operator,(T &&, X &) { return 0; }
X value;
X &f() { return (void(), value); }
