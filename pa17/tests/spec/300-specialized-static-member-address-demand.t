// N3485 focus: 14.7.3 [temp.expl.spec]/6: a member specialization owns its definition.
template<class T> struct mutable_value { static int value; };
template<class T> int mutable_value<T>::value = 3;
template<> int mutable_value<int>::value = 7;
template<class T> struct poison_value { static int value; };
template<class T> int poison_value<T>::value = T::missing;
template<> int poison_value<int>::value = 11;
template<class T> struct constant_value { static const int value; };
template<class T> const int constant_value<T>::value = 3;
template<> const int constant_value<int>::value = 13;
int* first = &mutable_value<int>::value;
int* second = &poison_value<int>::value;
const int* third = &constant_value<int>::value;
int main() { return *first != 7 || *second != 11 || *third != 13; }
