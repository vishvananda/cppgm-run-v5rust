// N3485 focus: 14.7.2 [temp.explicit]/8,9: explicit class definition demands defined static members.
template<class T> struct values { static int value; };
template<class T> int values<T>::value = T::missing;
template struct values<int>;
int main() { return 0; }
