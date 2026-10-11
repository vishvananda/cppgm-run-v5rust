// N3485 focus: 14.7.2 [temp.explicit]/3: an explicit static member definition requires its initializer.
template<class T> struct values { static int value; };
template<class T> int values<T>::value = T::missing;
template int values<int>::value;
int main() { return 0; }
