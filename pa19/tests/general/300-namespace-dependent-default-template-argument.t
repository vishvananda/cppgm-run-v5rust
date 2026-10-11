namespace N {

template<class T>
struct head;

template<class T, unsigned long S, bool = S <= sizeof(typename head<T>::type)>
struct X;

}

int main() {
  return 0;
}
