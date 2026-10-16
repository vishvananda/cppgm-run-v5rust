// expect exit 0
namespace N {
template<class T, class U, class = void> inline const bool value = false;
template<class T> inline const bool value<T, T> = true;
template<class T> inline const bool value<T*, T*> = sizeof(typename T::missing);
template<class T> extern int shape;
template<class T> int shape<const T> = 1;
template<class T> int shape<T[3]> = 2;
template<class T> int shape<T(T)> = 3;
}
template<class T> int N::shape<T&> = 4;
int main() { return 0; }
