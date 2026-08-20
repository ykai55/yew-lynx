import { Builder, ByteBuffer } from "flatbuffers";

import { AddEventListenerCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/add-event-listener-command.js";
import { AppendElementCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/append-element-command.js";
import { BooleanResult } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/boolean-result.js";
import { Channel } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/channel.js";
import { Command } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/command.js";
import { CommandBatch } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/command-batch.js";
import { CreateElementCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/create-element-command.js";
import { CreateRawTextCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/create-raw-text-command.js";
import { ElementCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/element-command.js";
import { Envelope } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/envelope.js";
import { EventMessage } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/event-message.js";
import { ElementIdResult } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/element-id-result.js";
import { ElementIdsResult } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/element-ids-result.js";
import { GetTagCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/get-tag-command.js";
import { GetClassesCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/get-classes-command.js";
import { InsertElementBeforeCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/insert-element-before-command.js";
import { Message } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/message.js";
import { NumberResult } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/number-result.js";
import { Payload } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/payload.js";
import { ReleaseElementCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/release-element-command.js";
import { RemoveElementCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/remove-element-command.js";
import { RemoveEventListenerCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/remove-event-listener-command.js";
import { ResponseBatch } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/response-batch.js";
import { ResultItem } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/result-item.js";
import { ResultValue } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/result-value.js";
import { ResultValueKind } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/result-value-kind.js";
import { SetAttributeCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/set-attribute-command.js";
import { SetStaticStyleCommand } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/set-static-style-command.js";
import { Status } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/status.js";
import { StringResult } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/string-result.js";
import { StringsResult } from "../../../protocol/generated/typescript/lynx/element-bridge/v2/strings-result.js";
import { decodeElementApiCommand } from "../../../protocol/generated/typescript/element_api_dispatch.js";

export const PROTOCOL_VERSION = 2;
const NULL_CONTENT_TYPE = "application/vnd.lynx-element-bridge.null";
const TEXT_CONTENT_TYPE = "text/plain;charset=utf-8";

type DomainOperation = Record<string, unknown> & { op: string };

export type DecodedEnvelope = {
  version: number;
  ok: boolean;
  status?: number;
  error?: string;
  operations: DomainOperation[];
  results?: HostResult[];
  event?: {
    listener: number;
    callback: number;
    contentType: string;
    payload: Uint8Array;
  };
  session: number;
  sequence: number;
};

export type HostResult = {
  slot: number;
  status: number;
  message?: string;
  resultKind: string;
  value?: unknown;
};

export function normalizeByteArray(input: unknown): Uint8Array {
  if (input instanceof Uint8Array) {
    return input;
  }
  if (input instanceof ArrayBuffer) {
    return new Uint8Array(input);
  }
  if (input === null || input === undefined) {
    throw new TypeError("response must be a ByteArray");
  }
  const length = (input as { length?: unknown }).length;
  if (!Number.isInteger(length) || (length as number) < 0) {
    throw new TypeError("response ByteArray has no readable length");
  }
  const bytes = new Uint8Array(length as number);
  for (let index = 0; index < bytes.length; index += 1) {
    const byte = (input as Record<number, unknown>)[index];
    if (!Number.isInteger(byte) || (byte as number) < 0 || (byte as number) > 255) {
      throw new TypeError(`response ByteArray byte ${index} is invalid`);
    }
    bytes[index] = byte as number;
  }
  return bytes;
}

export function decodeBridgeEnvelope(input: unknown, rootId: number): DecodedEnvelope {
  const bytes = normalizeByteArray(input);
  const buffer = new ByteBuffer(bytes);
  if (!Envelope.bufferHasIdentifier(buffer)) {
    throw new TypeError("response is not a LEB2 FlatBuffer");
  }
  const envelope = Envelope.getRootAsEnvelope(buffer);
  if (envelope.version() !== PROTOCOL_VERSION) {
    throw new TypeError(`response.version must be ${PROTOCOL_VERSION}`);
  }

  if (envelope.channel() === Channel.COMMAND && envelope.messageType() === Message.CommandBatch) {
    const batch = envelope.message(new CommandBatch()) as CommandBatch | null;
    if (batch === null || !batch.finalCommit()) {
      throw new TypeError("command batch must end at a final commit boundary");
    }
    const operations: DomainOperation[] = [];
    for (let index = 0; index < batch.commandsLength(); index += 1) {
      const command = batch.commands(index);
      if (command === null) {
        throw new TypeError(`command ${index} is missing`);
      }
      operations.push(decodeCommand(command));
    }
    operations.push({ op: "flush", root: rootId });
    return {
      version: PROTOCOL_VERSION,
      ok: true,
      operations,
      session: batch.sessionId(),
      sequence: batch.sequence(),
    };
  }

  if (envelope.channel() === Channel.RESULT && envelope.messageType() === Message.ResponseBatch) {
    const response = envelope.message(new ResponseBatch()) as ResponseBatch | null;
    if (response === null) {
      throw new TypeError("result envelope has no ResponseBatch");
    }
    return {
      version: PROTOCOL_VERSION,
      ok: response.status() === Status.OK,
      status: response.status(),
      error: response.message() || "native bridge failure",
      operations: [],
      results: Array.from(
        { length: response.resultsLength() },
        (_, index) => {
          const result = response.results(index);
          if (result === null) throw new TypeError(`result ${index} is missing`);
          return {
            slot: result.slot(),
            status: result.status(),
            message: result.message() || undefined,
            ...decodeHostResultValue(result),
          };
        },
      ),
      session: response.sessionId(),
      sequence: response.sequence(),
    };
  }

  if (envelope.channel() === Channel.EVENT && envelope.messageType() === Message.EventMessage) {
    const event = envelope.message(new EventMessage()) as EventMessage | null;
    if (event === null) {
      throw new TypeError("event envelope has no EventMessage");
    }
    return {
      version: PROTOCOL_VERSION,
      ok: true,
      operations: [],
      session: event.sessionId(),
      sequence: 0,
      event: {
        listener: event.listenerId(),
        callback: event.callbackId(),
        contentType: requiredString(event.contentType(), "EventMessage.contentType"),
        payload: event.payloadArray() || new Uint8Array(),
      },
    };
  }

  throw new TypeError("envelope channel and message do not match");
}

function decodeHostResultValue(result: ResultItem): Pick<HostResult, "resultKind" | "value"> {
  switch (result.valueKind()) {
    case ResultValueKind.NONE:
      return { resultKind: "void" };
    case ResultValueKind.ELEMENT_ID: {
      const value = result.value(new ElementIdResult()) as ElementIdResult | null;
      return { resultKind: "element_id", value: value?.value() };
    }
    case ResultValueKind.ELEMENT_IDS: {
      const value = result.value(new ElementIdsResult()) as ElementIdsResult | null;
      return {
        resultKind: "element_ids",
        value: Array.from({ length: value?.valuesLength() || 0 }, (_, index) => value?.values(index)),
      };
    }
    case ResultValueKind.STRING: {
      const value = result.value(new StringResult()) as StringResult | null;
      return { resultKind: "string", value: value?.value() };
    }
    case ResultValueKind.STRINGS: {
      const value = result.value(new StringsResult()) as StringsResult | null;
      return {
        resultKind: "strings",
        value: Array.from({ length: value?.valuesLength() || 0 }, (_, index) => value?.values(index)),
      };
    }
    case ResultValueKind.BOOLEAN: {
      const value = result.value(new BooleanResult()) as BooleanResult | null;
      return { resultKind: "boolean", value: value?.value() };
    }
    case ResultValueKind.NUMBER: {
      const value = result.value(new NumberResult()) as NumberResult | null;
      return { resultKind: "number", value: value?.value() };
    }
    case ResultValueKind.PAYLOAD: {
      const value = result.value(new Payload()) as Payload | null;
      return { resultKind: "payload", value: decodePayloadValue(value) };
    }
    default:
      throw new TypeError(`unsupported result kind ${result.valueKind()}`);
  }
}

function decodeCommand(command: Command): DomainOperation {
  switch (command.operationType()) {
    case ElementCommand.CreateElementCommand: {
      const operation = command.operation(new CreateElementCommand()) as CreateElementCommand | null;
      return {
        op: "create_element",
        node: command.resultNodeId(),
        tag: requiredString(operation?.tag(), "CreateElement.tag"),
      };
    }
    case ElementCommand.CreateRawTextCommand: {
      const operation = command.operation(new CreateRawTextCommand()) as CreateRawTextCommand | null;
      return {
        op: "create_text",
        node: command.resultNodeId(),
        text: requiredString(operation?.text(), "CreateRawText.text"),
      };
    }
    case ElementCommand.AppendElementCommand: {
      const operation = command.operation(new AppendElementCommand()) as AppendElementCommand | null;
      return { op: "insert_before", parent: operation?.parent(), child: operation?.current(), reference: null };
    }
    case ElementCommand.InsertElementBeforeCommand: {
      const operation = command.operation(new InsertElementBeforeCommand()) as InsertElementBeforeCommand | null;
      return {
        op: "insert_before",
        parent: operation?.parent(),
        child: operation?.current(),
        reference: operation?.marker(),
      };
    }
    case ElementCommand.RemoveElementCommand: {
      const operation = command.operation(new RemoveElementCommand()) as RemoveElementCommand | null;
      return { op: "remove", parent: operation?.parent(), child: operation?.current() };
    }
    case ElementCommand.ReleaseElementCommand: {
      const operation = command.operation(new ReleaseElementCommand()) as ReleaseElementCommand | null;
      return { op: "destroy_node", node: operation?.node() };
    }
    case ElementCommand.SetAttributeCommand: {
      const operation = command.operation(new SetAttributeCommand()) as SetAttributeCommand | null;
      const payload = operation?.value();
      return {
        op: "set_attribute",
        node: operation?.current(),
        name: requiredString(operation?.attrName(), "SetAttribute.attrName"),
        value: decodeAttribute(payload),
      };
    }
    case ElementCommand.AddEventListenerCommand: {
      const operation = command.operation(new AddEventListenerCommand()) as AddEventListenerCommand | null;
      return {
        op: "add_event_listener",
        node: operation?.node(),
        listener: command.listenerId(),
        callback: operation?.callback(),
        name: requiredString(operation?.name(), "AddEventListener.name"),
      };
    }
    case ElementCommand.RemoveEventListenerCommand: {
      const operation = command.operation(new RemoveEventListenerCommand()) as RemoveEventListenerCommand | null;
      return {
        op: "remove_event_listener",
        node: operation?.node(),
        listener: command.listenerId(),
        callback: operation?.callback(),
        name: requiredString(operation?.name(), "RemoveEventListener.name"),
      };
    }
    case ElementCommand.GetTagCommand: {
      const operation = command.operation(new GetTagCommand()) as GetTagCommand | null;
      return { op: "get_tag", node: operation?.node(), result_slot: command.resultSlot() };
    }
    default: {
      const operation = decodeElementApiCommand(command, decodePayloadValue, decodeReferences);
      return {
        op: "element_api",
        ...operation,
        result_slot: command.resultSlot(),
        result_node: command.resultNodeId(),
        result_nodes: Array.from(
          { length: command.resultNodeIdsLength() },
          (_, index) => command.resultNodeIds(index),
        ),
        listener: command.listenerId(),
      };
    }
  }
}

function requiredString(value: string | Uint8Array | null | undefined, field: string): string {
  if (typeof value !== "string") {
    throw new TypeError(`${field} is missing`);
  }
  return value;
}

function decodeAttribute(payload: Payload | null | undefined): string | null {
  if (payload === null || payload === undefined) {
    throw new TypeError("SetAttribute.value is missing");
  }
  const contentType = payload.contentType();
  if (contentType === NULL_CONTENT_TYPE) {
    return null;
  }
  if (contentType !== TEXT_CONTENT_TYPE) {
    throw new TypeError(`unsupported attribute content type ${contentType}`);
  }
  return new TextDecoder().decode(payload.bytesArray() || new Uint8Array());
}

function decodePayloadValue(payload: unknown): unknown {
  if (!(payload instanceof Payload)) {
    return null;
  }
  const contentType = payload.contentType();
  const bytes = payload.bytesArray() || new Uint8Array();
  if (contentType === NULL_CONTENT_TYPE) return null;
  if (contentType === TEXT_CONTENT_TYPE) return new TextDecoder().decode(bytes);
  if (contentType === "application/json") return JSON.parse(new TextDecoder().decode(bytes));
  return { contentType, bytes };
}

function decodeReferences(references: unknown): unknown {
  if (references === null || typeof references !== "object") return null;
  const value = references as {
    cardinality(): number;
    one(): number;
    manyLength(): number;
    many(index: number): number | null;
  };
  if (value.cardinality() === 0) return null;
  if (value.cardinality() === 1) return value.one();
  return Array.from({ length: value.manyLength() }, (_, index) => value.many(index));
}

export function encodeTestBatch(operations: DomainOperation[], root = 1): Uint8Array {
  const builder = new Builder(1024);
  const offsets = operations
    .filter((operation) => operation.op !== "flush")
    .map((operation) => encodeTestCommand(builder, operation));
  const commands = CommandBatch.createCommandsVector(builder, offsets);
  const batch = CommandBatch.createCommandBatch(builder, 1, 1, commands, true);
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    Channel.COMMAND,
    Message.CommandBatch,
    batch,
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  void root;
  return builder.asUint8Array();
}

export function encodeTestFailure(status: number, error: string): Uint8Array {
  const builder = new Builder(256);
  const message = builder.createString(error);
  const response = ResponseBatch.createResponseBatch(builder, 0, 0, status, message, 0, false);
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    Channel.RESULT,
    Message.ResponseBatch,
    response,
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  return builder.asUint8Array();
}

export function encodeHostResponse(
  session: number,
  sequence: number,
  results: HostResult[],
): ArrayBuffer {
  const builder = new Builder(512);
  const offsets = results.map((result) => encodeHostResult(builder, result));
  const resultVector = ResponseBatch.createResultsVector(builder, offsets);
  const response = ResponseBatch.createResponseBatch(
    builder,
    session,
    sequence,
    Status.OK,
    0,
    resultVector,
    true,
  );
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    Channel.RESULT,
    Message.ResponseBatch,
    response,
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  return exactArrayBuffer(builder.asUint8Array());
}

export function encodeHostEvent(
  session: number,
  listener: number,
  callback: number,
  eventData: unknown,
): ArrayBuffer {
  const builder = new Builder(256);
  const contentType = builder.createString("application/json");
  const payload = EventMessage.createPayloadVector(
    builder,
    new TextEncoder().encode(JSON.stringify(eventData ?? null)),
  );
  const event = EventMessage.createEventMessage(
    builder,
    session,
    listener,
    callback,
    contentType,
    payload,
  );
  const envelope = Envelope.createEnvelope(
    builder,
    PROTOCOL_VERSION,
    Channel.EVENT,
    Message.EventMessage,
    event,
  );
  Envelope.finishEnvelopeBuffer(builder, envelope);
  return exactArrayBuffer(builder.asUint8Array());
}

function exactArrayBuffer(bytes: Uint8Array): ArrayBuffer {
  return bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength);
}

function encodeHostResult(builder: Builder, result: HostResult): number {
  const message = result.message === undefined ? 0 : builder.createString(result.message);
  let valueKind = ResultValueKind.NONE;
  let valueType = ResultValue.NONE;
  let value = 0;
  if (result.status === Status.OK) {
    switch (result.resultKind) {
      case "void":
        break;
      case "element_id":
        valueKind = ResultValueKind.ELEMENT_ID;
        valueType = ResultValue.ElementIdResult;
        value = ElementIdResult.createElementIdResult(builder, result.value as number);
        break;
      case "element_ids": {
        valueKind = ResultValueKind.ELEMENT_IDS;
        valueType = ResultValue.ElementIdsResult;
        const values = ElementIdsResult.createValuesVector(builder, result.value as number[]);
        value = ElementIdsResult.createElementIdsResult(builder, values);
        break;
      }
      case "string": {
        valueKind = ResultValueKind.STRING;
        valueType = ResultValue.StringResult;
        const string = builder.createString(result.value as string);
        value = StringResult.createStringResult(builder, string);
        break;
      }
      case "strings": {
        valueKind = ResultValueKind.STRINGS;
        valueType = ResultValue.StringsResult;
        const strings = (result.value as string[]).map((item) => builder.createString(item));
        const values = StringsResult.createValuesVector(builder, strings);
        value = StringsResult.createStringsResult(builder, values);
        break;
      }
      case "boolean":
        valueKind = ResultValueKind.BOOLEAN;
        valueType = ResultValue.BooleanResult;
        value = BooleanResult.createBooleanResult(builder, result.value as boolean);
        break;
      case "number":
        valueKind = ResultValueKind.NUMBER;
        valueType = ResultValue.NumberResult;
        value = NumberResult.createNumberResult(builder, result.value as number);
        break;
      case "payload": {
        valueKind = ResultValueKind.PAYLOAD;
        valueType = ResultValue.Payload;
        const contentType = builder.createString("application/json");
        const bytes = Payload.createBytesVector(
          builder,
          new TextEncoder().encode(JSON.stringify(result.value)),
        );
        value = Payload.createPayload(builder, contentType, bytes);
        break;
      }
      default:
        throw new TypeError(`unsupported result kind ${result.resultKind}`);
    }
  }
  return ResultItem.createResultItem(
    builder,
    result.slot,
    result.status,
    message,
    valueKind,
    valueType,
    value,
  );
}

function encodeTestCommand(builder: Builder, operation: DomainOperation): number {
  let operationType: ElementCommand;
  let operationOffset: number;
  let resultNodeId = 0;
  let listenerId = 0;
  switch (operation.op) {
    case "create_element": {
      const tag = builder.createString(operation.tag as string);
      CreateElementCommand.startCreateElementCommand(builder);
      CreateElementCommand.addTag(builder, tag);
      operationOffset = CreateElementCommand.endCreateElementCommand(builder);
      operationType = ElementCommand.CreateElementCommand;
      resultNodeId = operation.node as number;
      break;
    }
    case "create_text": {
      const text = builder.createString(operation.text as string);
      CreateRawTextCommand.startCreateRawTextCommand(builder);
      CreateRawTextCommand.addText(builder, text);
      operationOffset = CreateRawTextCommand.endCreateRawTextCommand(builder);
      operationType = ElementCommand.CreateRawTextCommand;
      resultNodeId = operation.node as number;
      break;
    }
    case "insert_before": {
      if (operation.reference === null) {
        AppendElementCommand.startAppendElementCommand(builder);
        AppendElementCommand.addParent(builder, operation.parent as number);
        AppendElementCommand.addCurrent(builder, operation.child as number);
        operationOffset = AppendElementCommand.endAppendElementCommand(builder);
        operationType = ElementCommand.AppendElementCommand;
      } else {
        InsertElementBeforeCommand.startInsertElementBeforeCommand(builder);
        InsertElementBeforeCommand.addParent(builder, operation.parent as number);
        InsertElementBeforeCommand.addCurrent(builder, operation.child as number);
        InsertElementBeforeCommand.addMarker(builder, operation.reference as number);
        operationOffset = InsertElementBeforeCommand.endInsertElementBeforeCommand(builder);
        operationType = ElementCommand.InsertElementBeforeCommand;
      }
      break;
    }
    case "remove": {
      RemoveElementCommand.startRemoveElementCommand(builder);
      RemoveElementCommand.addParent(builder, operation.parent as number);
      RemoveElementCommand.addCurrent(builder, operation.child as number);
      operationOffset = RemoveElementCommand.endRemoveElementCommand(builder);
      operationType = ElementCommand.RemoveElementCommand;
      break;
    }
    case "destroy_node": {
      ReleaseElementCommand.startReleaseElementCommand(builder);
      ReleaseElementCommand.addNode(builder, operation.node as number);
      operationOffset = ReleaseElementCommand.endReleaseElementCommand(builder);
      operationType = ElementCommand.ReleaseElementCommand;
      break;
    }
    case "set_attribute": {
      const name = builder.createString(operation.name as string);
      const value = operation.value as string | null;
      const contentType = builder.createString(value === null ? NULL_CONTENT_TYPE : TEXT_CONTENT_TYPE);
      const bytes = value === null
        ? 0
        : Payload.createBytesVector(builder, new TextEncoder().encode(value));
      const payload = Payload.createPayload(builder, contentType, bytes);
      SetAttributeCommand.startSetAttributeCommand(builder);
      SetAttributeCommand.addCurrent(builder, operation.node as number);
      SetAttributeCommand.addAttrName(builder, name);
      SetAttributeCommand.addValue(builder, payload);
      operationOffset = SetAttributeCommand.endSetAttributeCommand(builder);
      operationType = ElementCommand.SetAttributeCommand;
      break;
    }
    case "add_event_listener": {
      const name = builder.createString(operation.name as string);
      AddEventListenerCommand.startAddEventListenerCommand(builder);
      AddEventListenerCommand.addNode(builder, operation.node as number);
      AddEventListenerCommand.addName(builder, name);
      AddEventListenerCommand.addCallback(builder, (operation.callback || operation.listener) as number);
      operationOffset = AddEventListenerCommand.endAddEventListenerCommand(builder);
      operationType = ElementCommand.AddEventListenerCommand;
      listenerId = operation.listener as number;
      break;
    }
    case "remove_event_listener": {
      const name = builder.createString((operation.name || "tap") as string);
      RemoveEventListenerCommand.startRemoveEventListenerCommand(builder);
      RemoveEventListenerCommand.addNode(builder, operation.node as number);
      RemoveEventListenerCommand.addName(builder, name);
      RemoveEventListenerCommand.addCallback(builder, (operation.callback || operation.listener) as number);
      operationOffset = RemoveEventListenerCommand.endRemoveEventListenerCommand(builder);
      operationType = ElementCommand.RemoveEventListenerCommand;
      listenerId = operation.listener as number;
      break;
    }
    case "get_tag": {
      GetTagCommand.startGetTagCommand(builder);
      GetTagCommand.addNode(builder, operation.node as number);
      operationOffset = GetTagCommand.endGetTagCommand(builder);
      operationType = ElementCommand.GetTagCommand;
      break;
    }
    case "get_classes": {
      operationOffset = GetClassesCommand.createGetClassesCommand(
        builder,
        operation.node as number,
      );
      operationType = ElementCommand.GetClassesCommand;
      break;
    }
    case "set_static_style": {
      SetStaticStyleCommand.startSetStaticStyleCommand(builder);
      SetStaticStyleCommand.addNode(builder, operation.node as number);
      SetStaticStyleCommand.addKey(builder, operation.key as number);
      operationOffset = SetStaticStyleCommand.endSetStaticStyleCommand(builder);
      operationType = ElementCommand.SetStaticStyleCommand;
      break;
    }
    default:
      throw new TypeError(`cannot encode test operation ${operation.op}`);
  }
  Command.startCommand(builder);
  if (operation.result_slot !== undefined) {
    Command.addResultSlot(builder, operation.result_slot as number);
  }
  if (resultNodeId !== 0) {
    Command.addResultNodeId(builder, resultNodeId);
  }
  if (listenerId !== 0) {
    Command.addListenerId(builder, listenerId);
  }
  Command.addOperationType(builder, operationType);
  Command.addOperation(builder, operationOffset);
  return Command.endCommand(builder);
}
