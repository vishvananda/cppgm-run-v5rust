namespace model { enum role { entry }; }
namespace internal { using namespace model; }
int select(internal::role value) {
  switch(value) { case internal::entry: return 1; }
  return 0;
}
