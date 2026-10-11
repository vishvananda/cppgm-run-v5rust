// N3485 focus: 14.7.2 [temp.explicit]/8,9: instantiate members defined at the class instantiation.
int calls;
int initialize() { ++calls; return 7; }
template<class T> struct explicit_value { static int value; };
template<class T> int explicit_value<T>::value = initialize();
template struct explicit_value<int>;
template<class T> struct dormant { static int value; };
template<class T> int dormant<T>::value = T::missing;
extern template struct dormant<int>;
int main() { return calls != 1; }
