// N3485 14.3.1p2 (C++11: local types may be template arguments) + 14.6: inside
// a function template, a local class L used as a template argument (Box<L>)
// names that class; expect exit 0
template<class T> struct Box { static int n; static int inc() { return ++n; } };
template<class T> int Box<T>::n;
template<class T> int box_in_template() { struct L { }; return Box<L>::inc(); }
int main() { return box_in_template<int>() - 1; }
