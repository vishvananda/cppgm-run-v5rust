int take(int *);
template<class T> int invalid() { return take(sizeof(T)); }
int main() { return 0; }
