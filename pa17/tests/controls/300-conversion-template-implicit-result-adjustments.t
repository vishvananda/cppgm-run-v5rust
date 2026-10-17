// N3485 14.8.2.3p2-p7, 14.8.2.4p6-p9: result adjustments and ordered
// qualification alternatives; reference/cv result ties; expect exit 0
int selected;
int value = 7;
int* pointer;
struct PointerResult {
    template<class T> operator T*() { selected = 1; return 0; }
    template<class T> operator T() { return T::unused; }
};
struct ReferenceResult {
    template<class T> operator T&() { selected = 2; return value; }
    template<class T> operator const T&() { selected = 3; return value; }
};
struct PointerReference {
    template<class T> operator T*&() { selected = 4; return pointer; }
};
struct PointerLevels {
    template<class T> operator T**() { selected = 5; return 0; }
};
struct ValueResult {
    template<class T> operator T() { selected = 6; return T(); }
};
struct OrdinaryResult {
    operator int*() { selected = 7; return 0; }
    template<class T> operator T*() { return T::unused; }
};
int main() {
    PointerResult p;
    ReferenceResult r;
    PointerReference pr;
    PointerLevels pl;
    ValueResult v;
    OrdinaryResult o;
    const int* cp = p;
    if (cp != 0 || selected != 1) return 1;
    const int& cr = r;
    if (&cr != &value || selected != 3) return 2;
    int* const& cpr = pr;
    if (&cpr != &pointer || selected != 4) return 3;
    const int* const* cpp = pl;
    if (cpp != 0 || selected != 5) return 4;
    const int& temporary = v;
    if (temporary != 0 || selected != 6) return 5;
    int* ordinary = o;
    if (ordinary != 0 || selected != 7) return 6;
    return 0;
}
