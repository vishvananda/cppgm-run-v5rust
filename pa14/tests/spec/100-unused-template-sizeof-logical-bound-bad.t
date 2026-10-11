template<class T> int invalid(int (&)[sizeof(T) || *sizeof(T)]);
int main() { return 0; }
