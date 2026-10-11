struct Value {
    int number;
};
typedef Value Alias;

struct Source {
    int number;
    template<class T> operator T() { return T{number}; }
    template<class T> operator T() const { return T{number + 10}; }
};

struct Address {
    int number;
    template<class T> operator T*() { return &number; }
};

struct Reference {
    int number;
    template<class T> operator T&() { return number; }
};

struct Base {
    template<class T> operator T() const { return T{19}; }
};
struct Derived : Base {
    template<class T> operator T() const { return T{23}; }
};

struct ReferenceResult {
    int number;
    template<class T> operator T() { return number; }
};

template<> ReferenceResult::operator int() { return number + 1; }

template<class T> T explicit_convert(const Source& source) {
    return source.operator T();
}

int main() {
    Source source = {7};
    const Source& view = source;
    Value first = source.operator Value();
    Value second = view.operator Alias();
    Value implicit = source;
    int implicit_integer = source;
    Value after_implicit = source.operator Alias();
    if (first.number != 7 || second.number != 17 || implicit.number != 7 ||
        implicit_integer != 7 || after_implicit.number != 7) return 1;
    if (source.operator int() != 7 || view.operator long() != 17 ||
        source.operator const int() != 7) return 2;
    if (explicit_convert<Value>(source).number != 17) return 3;
    Address address = {31};
    int* pointer = address.operator int*();
    if (pointer != &address.number || *pointer != 31) return 4;
    Reference reference = {42};
    int& bound = reference.operator int&();
    bound = 73;
    if (reference.number != 73) return 5;
    ReferenceResult result = {91};
    int& result_reference = result.operator int&();
    result_reference = 101;
    if (result.number != 101) return 6;
    if (result.operator int() != 102) return 7;
    Derived derived;
    return derived.Base::operator int() != 19 || derived.operator int() != 23;
}
