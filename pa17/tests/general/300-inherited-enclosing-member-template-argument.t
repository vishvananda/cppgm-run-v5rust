// VALIDATION: compile-pass
template<class> struct list {};
struct base { template<class> struct row {}; };
struct machine : base { struct table { typedef list<row<int> > type; }; };
int main() { return 0; }
