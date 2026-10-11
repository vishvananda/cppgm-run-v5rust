namespace n {
template<bool Flag, class Pointer>
struct iterator {};
}

template<bool Flag, class Pointer>
void range(n::iterator<Flag, Pointer> first,
           n::iterator<Flag, Pointer> last) {}
