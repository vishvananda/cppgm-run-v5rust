// N3485 14.3.2p5: integer zero cannot convert to a pointer pack element; expect reject
template<int*... P> struct Pointers;
Pointers<nullptr, 0>* invalid;
