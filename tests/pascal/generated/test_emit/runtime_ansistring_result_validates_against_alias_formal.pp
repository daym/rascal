unit cresstr;
interface
uses sysutils;
type
  TMsgStr = AnsiString;
  TPathStr = AnsiString;
procedure Message1(w : longint; const s1 : TMsgStr);
procedure Run(const fn : TPathStr);
implementation
procedure Message1(w : longint; const s1 : TMsgStr);
begin
end;
procedure Run(const fn : TPathStr);
begin
  Message1(1, ExtractFileName(fn));
end;
end.
