// N3485 14.1p4-p5: a declarator ellipsis declares a non-type parameter pack.
struct Owner {};
template<int*... P> struct Pointers;
template<int*...> struct Unnamed;
template<int&... R> struct References;
template<int Owner::*... M> struct Members;
template<int(*... F)(int)> struct Functions;
template<class T, T*... P> struct Dependent;
