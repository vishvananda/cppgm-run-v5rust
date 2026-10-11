template<class T> struct trait { static const bool value = true; };
template<class T> void validate();

int main() {
  if (trait<int>::value) {
    validate<int>();
  }
}
