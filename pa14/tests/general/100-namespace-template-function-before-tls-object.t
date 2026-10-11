namespace ns {

template<class T>
struct Helper {
  Helper() {}
};

template<class T>
int force(T value) {
  return value;
}

int anchor() {
  return force(1);
}

thread_local Helper<int> helper;

}

int main() {
  return ns::anchor();
}
