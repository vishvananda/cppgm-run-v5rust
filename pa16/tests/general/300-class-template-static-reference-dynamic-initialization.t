struct value { int number; };

template<class T>
T & get() { static T stored = { 7 }; return stored; }

template<class T>
struct holder { static T & instance; };

template<class T>
T & holder<T>::instance = get<T>();

int main() { return holder<value>::instance.number != 7; }
