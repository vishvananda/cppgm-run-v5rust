// Host-object ABI: a host constructor throwing inside a cppgm++ array new braced-init-list unwinds through the cppgm++ cleanup, which destroys the built elements and frees the storage.
