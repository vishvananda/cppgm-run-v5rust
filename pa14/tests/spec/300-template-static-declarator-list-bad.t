// N3485 focus: 14 [temp]/3: one declarator per template declaration.
template<class T> struct values { static int first, second; };
template<class T> int values<T>::first = 3, values<T>::second = 7;
int main() { return 0; }
