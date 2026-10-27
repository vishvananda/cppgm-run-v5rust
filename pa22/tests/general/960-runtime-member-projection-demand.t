// Runtime member conversions retain real class-path queries, including zero
// offsets and null values. Nonzero data/function adjustments preserve nulls.
struct Prefix { int prefix; };
struct Base { int value; int get() { return value; } };
struct Derived : Prefix, Base {};
using Data = int Base::*;
using DerivedData = int Derived::*;
using Function = int (Base::*)();
using DerivedFunction = int (Derived::*)();
DerivedData up(Data pointer) { return pointer; }
Data down(DerivedData pointer) { return static_cast<Data>(pointer); }
DerivedFunction up_function(Function pointer) { return pointer; }
Function down_function(DerivedFunction pointer) { return static_cast<Function>(pointer); }
int main() {
    Derived object;
    object.value = 7;
    Data data = &Base::value;
    Function function = &Base::get;
    DerivedData null_data = up(nullptr);
    DerivedFunction null_function = up_function(nullptr);
    if (null_data || null_function || down(null_data) || down_function(null_function)) return 1;
    DerivedData converted_data = up(data);
    DerivedFunction converted_function = up_function(function);
    return object.*converted_data + (object.*converted_function)() +
        object.*down(converted_data) + (object.*down_function(converted_function))() - 28;
}
