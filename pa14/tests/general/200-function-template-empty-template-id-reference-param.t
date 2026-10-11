template<class T = void>
struct Less {};

template<class Alg, class Type>
void dispatch(Type* first, Type* last, Less<>& comp) {
  Less<Type> local;
}

int main() { return 0; }
