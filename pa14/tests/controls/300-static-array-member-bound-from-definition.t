// N3485 3.9p6, 14.5.1.3: a class template's static array member declared T a[]
// takes the bound of its definition in the instance; expect exit 0
template<class T> struct B { static T a[]; };
template<class T> T B<T>::a[3] = { 1, 2, 3 };
int main() { return sizeof(B<int>::a) == 3 * sizeof(int) ? 0 : 1; }
