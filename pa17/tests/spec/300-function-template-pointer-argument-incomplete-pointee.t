// VALIDATION: compile-pass
struct n {};
template<class T> struct lazy { typedef typename T::missing type; };
struct call { template<class T> static void execute(T*) {} };
int main() { call::execute(static_cast<lazy<n>*>(0)); }
