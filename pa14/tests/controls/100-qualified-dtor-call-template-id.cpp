// N3485 12.4p13/3.4.5p3-4: an explicit destructor call
// q.Pair<int>::~Pair<int>() names the destructor of the class template
// specialization; expect exit 0
inline void* operator new(decltype(sizeof(0)), void* p) noexcept { return p; }
static int dtors;
template <class T> struct Pair { T a, b; ~Pair() { ++dtors; } };
int main() { Pair<int> q = { 1, 2 }; q.Pair<int>::~Pair<int>(); ::new ((void*)&q) Pair<int>(); return dtors == 1 ? 0 : 1; }
