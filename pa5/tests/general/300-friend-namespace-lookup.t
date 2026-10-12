namespace N{typedef int word;struct C{};}
struct C{friend int N::f(int C) noexcept(sizeof(C));};
