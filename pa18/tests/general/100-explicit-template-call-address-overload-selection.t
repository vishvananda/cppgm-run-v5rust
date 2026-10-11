template<class T> T first(T);
double second(double);
template<class T> T second(T);
template<class T> void pick(T (*)(T), T (*)(T)) {}
int main() { pick<double>(&first<double>, &second); }
