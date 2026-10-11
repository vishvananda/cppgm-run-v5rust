template<class T> struct pool {
  static int data;
  static int get() { return data; }
  struct nested { static int run() { return get(); } };
};

template<class T> int pool<T>::data = 7;

int main() { return pool<int>::nested::run() - 7; }
