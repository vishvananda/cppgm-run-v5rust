// HHC-057
template<class TypeList>
struct head;

template<class TypeList, int Size, bool = Size <= sizeof(typename head<TypeList>::type)>
struct find_first;

int main() { return 0; }
