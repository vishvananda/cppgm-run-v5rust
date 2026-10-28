// expect reject
// Written class-parameter checks precede demand of the omitted default.
struct A { ~A() = delete; };
A& get();
template<class T> int selected(A, int x = T::missing);
int main() { return selected<int>(get()); }
