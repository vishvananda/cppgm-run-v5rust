struct Types { using scalar = int; };
__typeof__(typename Types::scalar()) value;
template<class T> struct Box { __typeof__(typename T::scalar()) member; };
Box<Types> box;
static_assert(sizeof(value) == 4, "typename cast expression");
static_assert(sizeof(box) == 4, "dependent typename cast expression");
int main() { return 0; }
