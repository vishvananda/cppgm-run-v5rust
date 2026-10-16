// expect exit 0
template<class T> struct Early { ~Early(); };
template<class T> Early<T>::~Early() {}
template<class T> struct Late { ~Late(); };
int main() {
  Early<int> early;
  Late<char> late;
  return 0;
}
template<class T> Late<T>::~Late() {}
