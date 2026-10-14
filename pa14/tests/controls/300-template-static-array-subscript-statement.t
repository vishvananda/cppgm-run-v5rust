// N3485 6.8/14.6p2: in the statement TS<int>::arr[0] = 9; the qualified name
// denotes a static data member (not a type) of the specialization, so it is an
// expression statement; expect exit 0
template<class T> struct TS { static int arr[3]; };
template<class T> int TS<T>::arr[3] = { 1, 2, 3 };
int main() { TS<int>::arr[0] = 9; return TS<int>::arr[0] + TS<int>::arr[2] == 12 ? 0 : 1; }
