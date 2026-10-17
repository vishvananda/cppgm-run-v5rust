// N3485 14.1p11: a non-type parameter pack cannot have a default argument.
template<int*... P = nullptr> struct Pointers;
