// Completing an inert argument must not reopen cleanup below the try's floor.
struct Error {};
struct Member { Member(); ~Member(); };
struct Object { Member member; };
struct Profile { int values[6]; };
Profile profile();
Object make(const Profile&);
void evaluate() {
  Member outside;
  try {
    Object object=make(profile());
  } catch (const Error&) {}
}
