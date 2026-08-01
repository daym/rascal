unit u;
interface
type
  TNode = class end;
  TTempCreateNode = class;
  PTempInfo = ^TTempInfo;
  TTempInfo = record
    HookOnCopy : PTempInfo;
    Owner : TTempCreateNode;
  end;
  TTempCreateNode = class(TNode)
    TempInfo : PTempInfo;
  end;
implementation
end.
