// Binding a returned reference does not create a cleanup segment by itself.
struct Object { ~Object(); };
Object& factory();
Object& bind() {
  Object& object=factory();
  return object;
}
