unit cfileutl;
interface
uses sysutils;
function RunIt(const path: ansistring; flags: TExecuteFlags = []): longint;
implementation
function RunIt(const path: ansistring; flags: TExecuteFlags): longint;
begin
  RunIt := SysUtils.ExecuteProcess(path, 'arg', flags);
end;
procedure Demo;
begin
  RunIt('tool');
end;
end.
