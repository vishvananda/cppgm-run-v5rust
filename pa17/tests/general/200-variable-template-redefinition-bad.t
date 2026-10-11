// N3485 3.2p1, 14p1 + PA17 variable templates: a variable template defined
// twice in one translation unit is ill-formed; expect reject
template<class T> int v = 1;
template<class T> int v = 2;
int main() { return 0; }
