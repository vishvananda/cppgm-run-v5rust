// expect exit 0
// Hidden function-result bindings retain their selected classes until lowering.
struct Cell { int n; Cell(int value):n(value){} Cell(const Cell& other):n(other.n){} };
struct Left : Cell { Left():Cell(7){} };
struct Right : Cell { Right():Cell(11){} };
struct Repeated : Left, Right {};
struct RepeatedSource { Repeated value; operator Repeated&() { return value; } };
Cell repeated_copy(RepeatedSource& value) { return value; }
struct RepeatedPointerSource { Repeated* pointer; RepeatedPointerSource(Repeated* value):pointer(value){} operator Repeated*() { return pointer; } };
Cell* repeated_pointer(RepeatedPointerSource& value) { return value; }
struct MemberBase { int n; int get() { return n; } };
struct Padding { int pad; };
struct MemberDerived : Padding, MemberBase {};
struct Single : MemberBase {};
typedef int MemberBase::* DataMember;
typedef int MemberDerived::* DerivedData;
typedef int (MemberBase::* FunctionMember)();
typedef int (MemberDerived::* DerivedFunction)();
struct DataSource { DataMember member; DataSource(DataMember value):member(value){} operator DataMember() { return member; } };
struct FunctionSource { FunctionMember member; FunctionSource(FunctionMember value):member(value){} operator FunctionMember() { return member; } };
int MemberDerived::* data_result(DataSource& value) { return value; }
int Single::* zero_result(DataSource& value) { return value; }
DerivedFunction function_result(FunctionSource& value) { return value; }
struct DataReferenceSource { DataMember member; operator DataMember&() { return member; } };
void bind_reference_result(DataReferenceSource& source) { const DerivedData& value = source; (void)value; }
struct PointerSource { MemberDerived* pointer; PointerSource(MemberDerived* value):pointer(value){} operator MemberDerived*() { return pointer; } };
int main() {
  RepeatedSource source;
  Cell copied = repeated_copy(source);
  RepeatedPointerSource repeated(&source.value), null_repeated(nullptr);
  MemberDerived object; object.n = 9;
  Single single; single.n = 3;
  DataSource data(&MemberBase::n), null_data(nullptr);
  FunctionSource function(&MemberBase::get), null_function(nullptr);
  int MemberDerived::* member = data_result(data);
  DerivedFunction method = function_result(function);
  const DerivedData& member_temporary = data;
  const DerivedFunction& function_temporary = function;
  const DerivedData& null_temporary = null_data;
  PointerSource pointer_source(&object), null_pointer(nullptr);
  MemberBase*const& pointer_temporary = pointer_source;
  MemberBase*const& null_pointer_temporary = null_pointer;
  return copied.n != 7 || repeated_pointer(repeated)->n != 7 || repeated_pointer(null_repeated) != nullptr || object.*member != 9 || (object.*method)() != 9 ||
         single.*zero_result(data) != 3 || data_result(null_data) != nullptr ||
         function_result(null_function) != nullptr || object.*member_temporary != 9 ||
         (object.*function_temporary)() != 9 || null_temporary != nullptr ||
         pointer_temporary->n != 9 || null_pointer_temporary != nullptr;
}
