struct Deleted
{
  Deleted() = default;
  Deleted(const Deleted&) = delete;
  Deleted& operator=(const Deleted&) = delete;
};
struct NestedDeleted { Deleted member; };
struct NontrivialMember
{
  NontrivialMember() = default;
  NontrivialMember(const NontrivialMember&) {}
  NontrivialMember& operator=(const NontrivialMember&) { return *this; }
};
struct DeletedNontrivial
{
  NontrivialMember member;
  DeletedNontrivial() = default;
  DeletedNontrivial(const DeletedNontrivial&) = delete;
  DeletedNontrivial& operator=(const DeletedNontrivial&) = delete;
};
struct CopyOverloads
{
  CopyOverloads(const CopyOverloads&) = default;
  CopyOverloads(CopyOverloads&) {}
};
struct AssignmentOverloads
{
  AssignmentOverloads& operator=(const AssignmentOverloads&) = default;
  AssignmentOverloads& operator=(AssignmentOverloads&) { return *this; }
};
struct DefaultedOverloads
{
  DefaultedOverloads(const DefaultedOverloads&) = default;
  DefaultedOverloads(DefaultedOverloads&) = default;
};
struct DefaultedLater
{
  DefaultedLater(const DefaultedLater&);
};
static_assert(!__is_trivially_copyable(DefaultedLater), "first declaration is user-provided");
DefaultedLater::DefaultedLater(const DefaultedLater&) = default;
struct DeletedDestructor { ~DeletedDestructor() = delete; };
struct DefaultedDestructorLater { ~DefaultedDestructorLater(); };
static_assert(!__is_trivially_copyable(DefaultedDestructorLater), "destructor first declaration");
DefaultedDestructorLater::~DefaultedDestructorLater() = default;
struct VirtualDeleted
{
  VirtualDeleted(const VirtualDeleted&) = delete;
  VirtualDeleted& operator=(const VirtualDeleted&) = delete;
  virtual void f();
};
template<class T> struct NestedTemplate { T member; };

static_assert(__is_trivially_copyable(Deleted), "deleted declarations can be trivial");
static_assert(__is_trivially_copyable(NestedDeleted), "implicit deletion retains triviality");
static_assert(__is_trivially_copyable(NestedTemplate<Deleted>), "specialized member facts");
static_assert(!__is_constructible(Deleted, const Deleted&), "copy viability is separate");
static_assert(!__is_assignable(Deleted&, const Deleted&), "assignment viability is separate");
static_assert(!__is_trivially_copyable(DeletedNontrivial), "deleted does not imply trivial");
static_assert(!__is_trivially_copyable(CopyOverloads), "every copy declaration matters");
static_assert(!__is_trivially_copyable(AssignmentOverloads), "every assignment matters");
static_assert(__is_trivially_copyable(DefaultedOverloads), "both defaulted copies are trivial");
static_assert(!__is_trivially_copyable(DefaultedLater), "defaulted later is user-provided");
static_assert(__is_trivially_copyable(DeletedDestructor), "deleted destructor declaration");
static_assert(!__is_trivially_copyable(DefaultedDestructorLater), "defaulted later destructor");
static_assert(!__is_trivially_copyable(VirtualDeleted), "virtual class declarations are nontrivial");

int main() { return 0; }
