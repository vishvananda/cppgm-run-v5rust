// expect exit 0
// Crossing typed non-type partials stay unselected until definition demand.
template<class T, T Value> struct constant {};
template<class T> struct select {};
template<class T> struct select<constant<T, 0>> { typedef typename T::missing unused; };
template<short Value> struct select<constant<short, Value>> {};
select<constant<short, 0>> *pointer;
int main() { return pointer != 0; }
