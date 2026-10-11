// N3485 focus: 12 [special] out-of-class special member definitions
template<class T> struct Box { Box(); ~Box(); };
template<class T> Box<T>::Box() {}
template<class T> Box<T>::~Box() {}
