template<class T>
void suffixes(T& object, T* pointer) {
  object[{4,5,6}];
  object.template value<3>();
  pointer->template value<3>();
  object.~T();
  pointer->~T();
}
