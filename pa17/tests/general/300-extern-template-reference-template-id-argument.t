template<class T>
struct Less {};

template<class Compare, class Pointer>
void sort(char*, char*, Compare);

extern template void sort<Less<char>&, char*>(char*, char*, Less<char>&);

int main() { return 0; }
