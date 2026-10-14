// N3485 11.3p3: `friend T;` where T is instantiated as a non-class type (int)
// is ignored; expect exit 0
template<typename T> class R { friend T; int secret; public: R() : secret(5) {} };
R<int> ignored;
int main() { return 0; }
