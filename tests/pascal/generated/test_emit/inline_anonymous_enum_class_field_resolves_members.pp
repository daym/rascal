unit u;
interface
type
  tlinker = class
    libctype : (libc5, glibc2, glibc21, uclibc);
    procedure setup;
  end;
implementation
procedure tlinker.setup;
begin
  libctype := glibc21;
end;
end.
