# Proofs Manager
This is the Proofs Manager repository, an implementation of a versatile proofs manager.

The Proof Manager is an adaptable Proof Manager designed to assist in the creation of proofs from a PIL2 pilout-formatted file. It is designed to be used in conjunction with the [PIL2](https://github.com/0xPolygonHermez/pilcom) compiler.

# Setting Up a Configuration File for Your Software

The configuration file must be in json format and will contain essential settings and parameters necessary for the proper functioning of your software.

## Fields

Mandatory fields are essential settings that must be included in the configuration file for your software to run correctly. They provide crucial information required by the software to perform its core functions. Let's examine the mandatory fields required for your configuration file (* = mandatory fields):

- **name***: Specifies the name of the configuration.
- **pilout***: Specifies the path to the pilout binary file.
- **executors**: Specifies the configuration for executors. This field allows users to define custom executor configurations if needed.
- **prover**: Specifies the configuration for the prover. Users can customize the prover settings based on their requirements.
- **meta**: Specifies the meta configuration. This field enables users to include additional meta information if necessary.
- **debug**: Specifies whether debug mode is enabled. This field is optional and defaults to `false` if not specified.
- **only_check**: Specifies whether only check mode is enabled. This field is optional and defaults to `false` if not specified.

**meta**, **executors** and prover **fields**, can be specified either as a JSON object containing prover configuration settings directly or as a string representing a path filename to a separate JSON file containing prover configuration. Using a string path allows users to manage prover configurations separately, facilitating better organization and management of configuration data, especially for complex or extensive prover configurations.

## Example

Here's an example configuration file demonstrating the mandatory and optional fields:

```json
{
    "name": "MyConfig",
    "pilout": "my_pilout",
    "executors": "path_to_executors_config.json",
    "prover": {
        "variant": "stark",
        "lib": "mock_value"
    },
    "meta": {
        "mock_key": "mock_value"
    },
    "debug": true,
    "only_check": false
}
