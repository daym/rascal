unit options;
interface
uses globals;
procedure demo;
implementation
uses dos;
procedure demo;
var fpcdir : string;
begin
  fpcdir := FixPath(getenv('FPCDIR'), false);
end;
end.
